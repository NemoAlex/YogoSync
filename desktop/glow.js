// Shared optical approximation for the home screen and theme editor.
window.YogoGlow = {
  render(element, pixels) {
    const colors = pixels.flat();
    [...element.children].forEach((dot, i) => {
      const color = colors[i];
      dot.classList.toggle('is-lit', color.some(Boolean));
      dot.style.setProperty('--light', color.join(','));
    });
  }
};
