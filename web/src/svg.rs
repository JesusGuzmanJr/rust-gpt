use {
    maud::{Markup, html},
    std::borrow::Cow,
};

pub(crate) fn plus(width: u32, height: u32) -> Markup {
    html! {
        svg.icon width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M12 5v14M5 12h14";
        }
    }
}

pub(crate) fn chat_bubble(width: u32, height: u32) -> Markup {
    html! {
        svg.icon.icon--sm width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z";
        }
    }
}

pub(crate) fn info_circle(class: &'static str, width: u32, height: u32) -> Markup {
    html! {
        svg class=(class) width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            circle cx="12" cy="12" r="10" {}
            path d="M12 16v-4M12 8h.01" {}
        }
    }
}

pub(crate) fn check_circle(class: &'static str, width: u32, height: u32) -> Markup {
    html! {
        svg class=(class) width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            circle cx="12" cy="12" r="10" {}
            path d="m9 12 2 2 4-4" {}
        }
    }
}

pub(crate) fn x_circle(class: &'static str, width: u32, height: u32) -> Markup {
    html! {
        svg class=(class) width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            circle cx="12" cy="12" r="10" {}
            path d="m15 9-6 6M9 9l6 6" {}
        }
    }
}

pub(crate) fn thumbs_up(width: u32, height: u32) -> Markup {
    html! {
        svg.icon.icon--sm width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3";
        }
    }
}

pub(crate) fn thumbs_down(width: u32, height: u32) -> Markup {
    html! {
        svg.icon.icon--sm width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17";
        }
    }
}

pub(crate) fn settings(width: u32, height: u32) -> Markup {
    html! {
        svg.icon width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" {}
            circle cx="12" cy="12" r="3" {}
        }
    }
}

pub(crate) fn arrow_right(width: u32, height: u32, stroke_width: u32) -> Markup {
    html! {
        svg.icon width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width=(stroke_width) {
            path d="M5 12h14M12 5l7 7-7 7";
        }
    }
}

pub(crate) fn edit(width: u32, height: u32) -> Markup {
    html! {
        svg.icon.icon--sm width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z";
            path d="m15 5 4 4";
        }
    }
}

pub(crate) fn user(width: u32, height: u32) -> Markup {
    html! {
        svg.input-icon width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" {}
            circle cx="12" cy="7" r="4" {}
        }
    }
}

pub(crate) fn envelope(class: &'static str, width: u32, height: u32) -> Markup {
    html! {
        svg class=(class) width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            rect x="3" y="5" width="18" height="14" rx="2" {}
            path d="m3 7 9 6 9-6" {}
        }
    }
}

pub(crate) fn lock(width: u32, height: u32) -> Markup {
    html! {
        svg.input-icon width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {
            rect x="5" y="11" width="14" height="10" rx="2" ry="2" {}
            path d="M7 11V7a5 5 0 0 1 10 0v4" {}
        }
    }
}

pub(crate) fn home(width: u32, height: u32) -> Markup {
    html! {
        svg width=(width) height=(height) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
            path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z";
            polyline points="9 22 9 12 15 12 15 22";
        }
    }
}
