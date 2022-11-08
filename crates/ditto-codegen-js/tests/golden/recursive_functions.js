function cycle_a() {
  return cycle_b();
}
function cycle_b() {
  return cycle_a();
}
function forever() {
  return forever();
}
export { cycle_a, cycle_b, forever };
