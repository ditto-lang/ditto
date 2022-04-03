function cycleA() {
  return cycleB();
}
function cycleB() {
  return cycleA();
}
function forever() {
  return forever();
}
export { cycleA, cycleB, forever };
