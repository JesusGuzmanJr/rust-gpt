use {
    crate::{job::Job, thread::ThreadId, user::UserId},
    chrono::{DateTime, Utc},
    std::collections::HashMap,
};

pub(crate) struct RoundRobin {
    user_queues: HashMap<UserId, HashMap<ThreadId, (Job, DateTime<Utc>)>>,
    order: Vec<UserId>, // round-robin over users
    idx: usize,
}

impl RoundRobin {
    pub(crate) fn new() -> Self {
        Self {
            user_queues: HashMap::new(),
            order: Vec::new(),
            idx: 0,
        }
    }

    pub(crate) fn push(&mut self, user: UserId, thread: ThreadId, job: Job) {
        if !self.user_queues.contains_key(&user) {
            self.order.push(user);
        }

        let user_queue = self.user_queues.entry(user).or_default();

        user_queue.insert(thread, (job, Utc::now()));
    }

    pub(crate) fn pop(&mut self) -> Option<(UserId, ThreadId, Job)> {
        if self.order.is_empty() {
            return None;
        }

        let user_id = self.order[self.idx];
        let user_queue = self.user_queues.get_mut(&user_id).expect("user not found");

        let thread_id = {
            let mut user_queue_vec = user_queue.iter().collect::<Vec<_>>();
            user_queue_vec.sort_by_key(|(_, (_, created_at))| *created_at);
            let (thread_id, _) = user_queue_vec.first().expect("job not found");

            **thread_id
        };
        let (job, _) = user_queue.remove(&thread_id).expect("job not found");

        if user_queue.is_empty() {
            self.user_queues.remove(&user_id);
            self.order.remove(self.idx);

            if !self.order.is_empty() {
                self.idx %= self.order.len();
            } else {
                self.idx = 0;
            }
        } else {
            self.idx = (self.idx + 1) % self.order.len();
        }

        Some((user_id, thread_id, job))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_scheduler_is_empty() {
        let mut scheduler = RoundRobin::new();
        assert_eq!(scheduler.pop(), None);
    }

    #[test]
    fn test_push_and_pop_single_job() {
        let mut scheduler = RoundRobin::new();
        let user_id = UserId::new();
        let thread_id = ThreadId::new();
        let job = Job {};

        scheduler.push(user_id, thread_id, job);

        let result = scheduler.pop();
        assert!(result.is_some());
        let (popped_user, popped_thread, _) = result.unwrap();
        assert_eq!(popped_user, user_id);
        assert_eq!(popped_thread, thread_id);
    }

    #[test]
    fn test_pop_empty_after_consuming_all_jobs() {
        let mut scheduler = RoundRobin::new();
        let user_id = UserId::new();
        let thread_id = ThreadId::new();
        let job = Job {};

        scheduler.push(user_id, thread_id, job);
        scheduler.pop();

        assert_eq!(scheduler.pop(), None);
    }

    #[test]
    fn test_multiple_jobs_same_user_fifo_order() {
        let mut scheduler = RoundRobin::new();
        let user_id = UserId::new();
        let thread_id_1 = ThreadId::new();
        let thread_id_2 = ThreadId::new();
        let thread_id_3 = ThreadId::new();

        // Push jobs in order
        scheduler.push(user_id, thread_id_1, Job {});
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_id, thread_id_2, Job {});
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_id, thread_id_3, Job {});

        // Should pop in FIFO order (earliest first)
        let (_, thread_1, _) = scheduler.pop().unwrap();
        assert_eq!(thread_1, thread_id_1);

        let (_, thread_2, _) = scheduler.pop().unwrap();
        assert_eq!(thread_2, thread_id_2);

        let (_, thread_3, _) = scheduler.pop().unwrap();
        assert_eq!(thread_3, thread_id_3);
    }

    #[test]
    fn test_round_robin_between_users() {
        let mut scheduler = RoundRobin::new();
        let user_1 = UserId::new();
        let user_2 = UserId::new();
        let user_3 = UserId::new();

        let thread_1 = ThreadId::new();
        let thread_2 = ThreadId::new();
        let thread_3 = ThreadId::new();

        // Push jobs for different users
        scheduler.push(user_1, thread_1, Job {});
        scheduler.push(user_2, thread_2, Job {});
        scheduler.push(user_3, thread_3, Job {});

        // Should pop in round-robin order
        let (popped_user_1, ..) = scheduler.pop().unwrap();
        let (popped_user_2, ..) = scheduler.pop().unwrap();
        let (popped_user_3, ..) = scheduler.pop().unwrap();

        assert_eq!(popped_user_1, user_1);
        assert_eq!(popped_user_2, user_2);
        assert_eq!(popped_user_3, user_3);
    }

    #[test]
    fn test_round_robin_wraps_around() {
        let mut scheduler = RoundRobin::new();
        let user_1 = UserId::new();
        let user_2 = UserId::new();

        let thread_1a = ThreadId::new();
        let thread_1b = ThreadId::new();
        let thread_2a = ThreadId::new();
        let thread_2b = ThreadId::new();

        // Push multiple jobs for each user
        scheduler.push(user_1, thread_1a, Job {});
        scheduler.push(user_1, thread_1b, Job {});
        scheduler.push(user_2, thread_2a, Job {});
        scheduler.push(user_2, thread_2b, Job {});

        // Should alternate between users in round-robin fashion
        let (user, ..) = scheduler.pop().unwrap();
        assert_eq!(user, user_1);

        let (user, ..) = scheduler.pop().unwrap();
        assert_eq!(user, user_2);

        let (user, ..) = scheduler.pop().unwrap();
        assert_eq!(user, user_1); // Wrapped around back to user_1

        let (user_d, ..) = scheduler.pop().unwrap();
        assert_eq!(user_d, user_2); // Back to user_2
    }

    #[test]
    fn test_same_thread_overwrite() {
        let mut scheduler = RoundRobin::new();
        let user_id = UserId::new();
        let thread_id = ThreadId::new();

        // Push same thread twice - should overwrite
        scheduler.push(user_id, thread_id, Job {});
        scheduler.push(user_id, thread_id, Job {});

        // Should only have one job
        assert!(scheduler.pop().is_some());
        assert_eq!(scheduler.pop(), None);
    }

    #[test]
    fn test_round_robin_with_multiple_users_and_threads() {
        let mut scheduler = RoundRobin::new();
        let user_1 = UserId::new();
        let user_2 = UserId::new();
        let user_3 = UserId::new();

        let u1_t1 = ThreadId::new();
        let u1_t2 = ThreadId::new();
        let u2_t1 = ThreadId::new();
        let u2_t2 = ThreadId::new();
        let u3_t1 = ThreadId::new();
        let u3_t2 = ThreadId::new();

        // User 1: 2 threads
        scheduler.push(user_1, u1_t1, Job {});
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_1, u1_t2, Job {});

        // User 2: 2 threads
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_2, u2_t1, Job {});
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_2, u2_t2, Job {});

        // User 3: 2 threads
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_3, u3_t1, Job {});
        std::thread::sleep(std::time::Duration::from_millis(10));
        scheduler.push(user_3, u3_t2, Job {});

        // Should alternate users in round-robin, with FIFO per user
        // Round 1: user_1 gets thread 1 (oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_1);
        assert_eq!(thread, u1_t1);

        // Round 1: user_2 gets thread 1 (oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_2);
        assert_eq!(thread, u2_t1);

        // Round 1: user_3 gets thread 1 (oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_3);
        assert_eq!(thread, u3_t1);

        // Round 2: user_1 gets thread 2 (next oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_1);
        assert_eq!(thread, u1_t2);

        // Round 2: user_2 gets thread 2 (next oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_2);
        assert_eq!(thread, u2_t2);

        // Round 2: user_3 gets thread 2 (next oldest)
        let (user, thread, _) = scheduler.pop().unwrap();
        assert_eq!(user, user_3);
        assert_eq!(thread, u3_t2);

        // All jobs processed
        assert_eq!(scheduler.pop(), None);
    }
}
