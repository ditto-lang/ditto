function Triple($0, $1, $2) {
  return ["Triple", $0, $1, $2];
}
function Wrapper($0) {
  return ["Wrapper", $0];
}
function curried_wrappers_to_triple($0) {
  if ($0[0] === "Wrapper") {
    const a = $0[1];
    return $0 => {
      if ($0[0] === "Wrapper") {
        const b = $0[1];
        return $0 => {
          if ($0[0] === "Wrapper") {
            const c = $0[1];
            return Triple(a, b, c);
          }
          throw new Error("Pattern match error");
        };
      }
      throw new Error("Pattern match error");
    };
  }
  throw new Error("Pattern match error");
}
function unwrap_int($0) {
  if ($0[0] === "Wrapper") {
    const i = $0[1];
    return i;
  }
  throw new Error("Pattern match error");
}
const five = unwrap_int(Wrapper(5));
function unwrap($0) {
  if ($0[0] === "Wrapper") {
    const a = $0[1];
    return a;
  }
  throw new Error("Pattern match error");
}
function wrappers_to_triple($0, wrapped_b, $2) {
  if ($0[0] === "Wrapper" && $2[0] === "Wrapper") {
    const c = $2[1];
    const a = $0[1];
    return Triple(a, unwrap(wrapped_b), c);
  }
  throw new Error("Pattern match error");
}
export {
  Triple,
  Wrapper,
  curried_wrappers_to_triple,
  five,
  unwrap,
  unwrap_int,
  wrappers_to_triple,
};
