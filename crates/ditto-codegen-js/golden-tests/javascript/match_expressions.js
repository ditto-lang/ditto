function Just($0) {
  return ["Just", $0];
}
function ManyFields($0, $1, $2, $3) {
  return ["ManyFields", $0, $1, $2, $3];
}
const Nothing = ["Nothing"];
function many_fields_to_array(mf) {
  return mf[0] === "ManyFields"
    ? (() => {
        const d = mf[4];
        const c = mf[3];
        const b = mf[2];
        const a = mf[1];
        return [a, b, c, d];
      })()
    : (() => {
        throw new Error("Pattern match error");
      })();
}
function mk_five(five) {
  return five[0] === "Five"
    ? 5
    : (() => {
        throw new Error("Pattern match error");
      })();
}
function with_default(maybe, $default) {
  return maybe[0] === "Nothing"
    ? $default
    : maybe[0] === "Just"
    ? (() => {
        const a = maybe[1];
        return a;
      })()
    : (() => {
        throw new Error("Pattern match error");
      })();
}
export {
  Just,
  ManyFields,
  Nothing,
  many_fields_to_array,
  mk_five,
  with_default,
};
