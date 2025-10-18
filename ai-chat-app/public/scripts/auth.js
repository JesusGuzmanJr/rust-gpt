document.addEventListener("DOMContentLoaded", () => {
  const signInForm = document.getElementById("signInForm")
  const signUpForm = document.getElementById("signUpForm")

  if (signInForm) {
    signInForm.addEventListener("submit", (e) => {
      e.preventDefault()
      // Simulate sign in
      window.location.href = "/index.html"
    })
  }

  if (signUpForm) {
    signUpForm.addEventListener("submit", (e) => {
      e.preventDefault()
      const password = document.getElementById("password").value
      const confirmPassword = document.getElementById("confirmPassword").value

      if (password !== confirmPassword) {
        alert("Passwords do not match")
        return
      }

      // Simulate sign up
      window.location.href = "/index.html"
    })
  }
})
