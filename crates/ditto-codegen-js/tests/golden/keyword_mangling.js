function $Error($0) {
  return ["Error", $0];
}
function unwrap_error($0) {
  if ($0[0] === "Error") {
    const msg = $0[1];
    return msg;
  }
  throw new Error("Pattern match error");
}
function $const(a) {
  return b => a;
}
export { $Error, $const, unwrap_error };
