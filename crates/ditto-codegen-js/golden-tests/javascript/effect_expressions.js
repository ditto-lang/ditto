function Err($0) {
  return ["Err", $0];
}
function Ok($0) {
  return ["Ok", $0];
}
function always(_a, b) {
  return b;
}
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
  return effect_map(get_name, name => undefined)();
}
function get_names() {
  const name = get_name();
  const another_name = get_name();
  return [name, another_name];
}
function get_names_from_result(res) {
  return () => {
    get_name();
    return (
      res[0] === "Ok"
        ? (() => {
            const a = res[1];
            return always(a, get_names);
          })()
        : res[0] === "Err"
        ? (() => {
            const e = res[1];
            return always(e, get_names);
          })()
        : (() => {
            throw new Error("Pattern match error");
          })()
    )();
  };
}
export {
  Err,
  Ok,
  always,
  effect_map,
  get_name,
  get_names,
  get_names_from_result,
  main,
};
