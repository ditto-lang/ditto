function effect_map(effect_a, fn) {
  return () => {
    const a = effect_a();
    return fn(a);
  };
}
function get_name() {
  return "jane";
}
function main() {
  effect_map(get_name, name => undefined)();
}
function get_names() {
  const name = get_name();
  const another_name = get_name();
  return [name, another_name];
}
export { effect_map, get_name, get_names, main };
