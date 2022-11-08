function Err($0) {
  return ["Err", $0];
}
function Ok($0) {
  return ["Ok", $0];
}
function Wrapped($0) {
  return ["Wrapped", $0];
}
function get_wrapped_five() {
  const five = 5;
  return Wrapped(five);
}
function get_five() {
  const wrapped_five = get_wrapped_five();
  const $0 = wrapped_five;
  if ($0[0] === "Wrapped") {
    const $1 = wrapped_five;
    if ($1[0] === "Wrapped") {
      const $2 = wrapped_five;
      if ($2[0] === "Wrapped") {
        const five = $2[1];
        return five;
      }
      throw new Error("Pattern match error");
    }
    throw new Error("Pattern match error");
  }
  throw new Error("Pattern match error");
}
function always(_a, b) {
  return b;
}
function effect_map(effect_a, f) {
  return () => {
    const a = effect_a();
    return f(a);
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
    return (() => {
      if (res[0] === "Ok") {
        const a = res[1];
        return always(a, get_names);
      }
      if (res[0] === "Err") {
        const e = res[1];
        return always(e, get_names);
      }
      throw new Error("Pattern match error");
    })()();
  };
}
export {
  Err,
  Ok,
  Wrapped,
  always,
  effect_map,
  get_five,
  get_name,
  get_names,
  get_names_from_result,
  get_wrapped_five,
  main,
};
