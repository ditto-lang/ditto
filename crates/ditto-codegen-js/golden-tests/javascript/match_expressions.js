function Just($0) {
  return ["Just", $0];
}
function ManyFields($0, $1, $2, $3) {
  return ["ManyFields", $0, $1, $2, $3];
}
const Nothing = ["Nothing"];
function is_just(maybe) {
  if (maybe[0] === "Just") {
    return true;
  }
  if (maybe[0] === "Nothing") {
    return false;
  }
  throw new Error("Pattern match error");
}
function many_fields_to_array(mf) {
  if (mf[0] === "ManyFields") {
    const d = mf[4];
    const c = mf[3];
    const b = mf[2];
    const a = mf[1];
    return [a, b, c, d];
  }
  throw new Error("Pattern match error");
}
function mk_five(five) {
  if (five[0] === "Five") {
    return 5;
  }
  throw new Error("Pattern match error");
}
function with_default(maybe, $default) {
  if (maybe[0] === "Just") {
    const a = maybe[1];
    return a;
  }
  if (maybe[0] === "Nothing") {
    return $default;
  }
  throw new Error("Pattern match error");
}
export {
  Just,
  ManyFields,
  Nothing,
  is_just,
  many_fields_to_array,
  mk_five,
  with_default,
};
