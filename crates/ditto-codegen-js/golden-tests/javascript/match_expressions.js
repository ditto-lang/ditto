function Just($0) {
  return ["Just", $0];
}
const Nothing = ["Nothing"];
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
export { Just, Nothing, mk_five, with_default };
