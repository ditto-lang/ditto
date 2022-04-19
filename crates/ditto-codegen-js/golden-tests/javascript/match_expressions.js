function Just($0) {
  return ["Just", $0];
}
const Nothing = ["Nothing"];
function withDefault(maybe, $default) {
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
export { Just, Nothing, withDefault };
