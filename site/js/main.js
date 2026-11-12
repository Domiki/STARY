// Mobile menu, scroll reveal, copy buttons and the star-trail hero.

const menuButton = document.querySelector('.menu-toggle')
const nav = document.querySelector('.nav')
menuButton?.addEventListener('click', () => {
  const open = nav.classList.toggle('open')
  menuButton.setAttribute('aria-expanded', String(open))
})

const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches

// ---------- reveal on scroll ----------
const revealed = document.querySelectorAll('.reveal')
if ('IntersectionObserver' in window && !reduceMotion) {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        entry.target.classList.add('visible')
        observer.unobserve(entry.target)
      }
    },
    { threshold: 0.12 },
  )
  revealed.forEach((element) => observer.observe(element))
} else {
  revealed.forEach((element) => element.classList.add('visible'))
}

// ---------- copy buttons on code blocks ----------
document.querySelectorAll('pre').forEach((block) => {
  const button = document.createElement('button')
  button.type = 'button'
  button.className = 'copy-button'
  button.textContent = 'Copy'
  button.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(block.querySelector('code')?.textContent ?? '')
      button.textContent = 'Copied'
    } catch {
      button.textContent = 'Press ⌘/Ctrl+C'
    }
    setTimeout(() => (button.textContent = 'Copy'), 1800)
  })
  block.append(button)
})

// ---------- star-trail hero ----------
// Stars turn about a pole that sits on the right of the text. Every frame the canvas is cleared and each
// star is drawn with a trail of the path it took in the last few seconds, fading toward its tail, so a
// trail always disappears after TRAIL_MS.
const canvas = document.querySelector('.hero canvas')
if (canvas) {
  const context = canvas.getContext('2d')
  const colors = ['224, 228, 233', '216, 164, 143', '154, 165, 180']
  const STARS = 260
  const SPEED = 0.00012
  const TRAIL_MS = 5000
  const PIECES = 14
  let width = 0
  let height = 0
  let pole = { x: 0, y: 0 }
  let stars = []
  let running = false
  let last = 0

  const makeStars = () =>
    Array.from({ length: STARS }, () => {
      const reach = Math.hypot(width, height) * 0.9
      return {
        radius: 20 + Math.random() ** 1.4 * reach,
        angle: Math.random() * Math.PI * 2,
        size: 0.5 + Math.random() * 1.4,
        color: colors[Math.floor(Math.random() * colors.length)],
        alpha: 0.35 + Math.random() * 0.65,
      }
    })

  const turnPerMs = (star) => SPEED * (1 + 140 / (star.radius + 140))

  const resize = () => {
    const ratio = Math.min(window.devicePixelRatio || 1, 2)
    width = canvas.clientWidth
    height = canvas.clientHeight
    canvas.width = width * ratio
    canvas.height = height * ratio
    context.setTransform(ratio, 0, 0, ratio, 0, 0)
    pole = { x: width * 0.76, y: height * 0.38 }
    stars = makeStars()
    draw()
  }

  const draw = () => {
    context.fillStyle = '#0b111b'
    context.fillRect(0, 0, width, height)
    context.lineCap = 'round'
    for (const star of stars) {
      const length = turnPerMs(star) * TRAIL_MS
      context.lineWidth = star.size
      for (let piece = 0; piece < PIECES; piece++) {
        const fade = (1 - piece / PIECES) ** 2
        context.strokeStyle = `rgba(${star.color}, ${star.alpha * fade})`
        context.beginPath()
        context.arc(
          pole.x,
          pole.y,
          star.radius,
          star.angle - (length * (piece + 1)) / PIECES,
          star.angle - (length * piece) / PIECES,
        )
        context.stroke()
      }
    }
  }

  const frame = (now) => {
    if (!running) return
    const elapsed = Math.min(now - last, 48)
    last = now
    for (const star of stars) star.angle += turnPerMs(star) * elapsed
    draw()
    requestAnimationFrame(frame)
  }

  resize()
  window.addEventListener('resize', resize)

  if (!reduceMotion) {
    new IntersectionObserver((entries) => {
      running = entries.some((entry) => entry.isIntersecting)
      if (running) {
        last = performance.now()
        requestAnimationFrame(frame)
      }
    }).observe(canvas)
  }
}
