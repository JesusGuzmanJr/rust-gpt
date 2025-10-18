document.addEventListener("DOMContentLoaded", () => {
  const landingInput = document.getElementById("landingInput")
  const startBtn = document.getElementById("startBtn")

  startBtn.addEventListener("click", () => {
    window.location.href = "/index.html"
  })

  landingInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      window.location.href = "/index.html"
    }
  })
})
