document.getElementById('guest-form').addEventListener('submit', (event) => {
  event.preventDefault()
  const url = document.getElementById('guest-url').value
  const expiration = document.getElementById('guest-expiration').value + 'd'
  const submitEl = document.getElementById('guest-submit')
  const outputContainerEl = document.getElementById('guest-output-container')
  const outputEl = document.getElementById('guest-output')
  const errorEl = document.getElementById('guest-error')

  submitEl.disabled = true
  hide(outputContainerEl)
  hide(errorEl)

  fetch('api/v1/guest-link', {
    method: 'POST', headers: {
      'Content-Type': 'application/json'
    }, body: JSON.stringify({
      url, expiration
    })
  }).then(response => {
    if (!response.ok) {
      throw new Error(response.statusText)
    }
    return response.json()
  }).then(response => {
    show(outputContainerEl)
    outputEl.textContent = response.url
  }).catch(error => {
    show(errorEl)
    console.error(error)
  }).finally(() => {
    submitEl.disabled = false
  })
})

function hide(el) {
  el.style.display = 'none'
}

function show(el) {
  el.style.display = ''
}