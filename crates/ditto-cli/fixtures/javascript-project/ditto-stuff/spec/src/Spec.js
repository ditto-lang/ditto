export function test_impl(name, effect) {
  return () => {
    console.log(name);
    effect();
  }
}
