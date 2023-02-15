const A = ["A"];
const B = ["B"];
const C = ["C"];
function Just($0) {
  return ["Just", $0];
}
function ManyFields($0, $1, $2, $3) {
  return ["ManyFields", $0, $1, $2, $3];
}
const Nothing = ["Nothing"];
function nested_pattern(m) {
  if (
    m[0] === "Just" &&
    m[1][0] === "Just" &&
    m[1][1][0] === "Just" &&
    m[1][1][1][0] === "Just"
  ) {
    return true;
  }
  if (m[0] === "Nothing") {
    return false;
  }
  return false;
}
function id(a) {
  return a;
}
function complex_matched_expresion(x) {
  const $0 = id(id(id(id(id(id(x))))));
  if ($0[0] === "Just") {
    const y = $0[1];
    return [y];
  }
  if ($0[0] === "Nothing") {
    return [2];
  }
  throw new Error("Pattern match error");
}
function to_string(abc) {
  if (abc[0] === "A") {
    return "A";
  }
  if (abc[0] === "B") {
    return "C";
  }
  if (abc[0] === "C") {
    return "C";
  }
  throw new Error("Pattern match error");
}
function effect_arms(maybe) {
  if (maybe[0] === "Just") {
    const a = maybe[1];
    return () => a;
  }
  if (maybe[0] === "Nothing") {
    return () => 5;
  }
  throw new Error("Pattern match error");
}
function very_function_arms(maybe) {
  if (maybe[0] === "Just") {
    const a = maybe[1];
    return b => c => d => [a, b, c];
  }
  if (maybe[0] === "Nothing") {
    return b => c => d => [1, b, c];
  }
  throw new Error("Pattern match error");
}
function function_arms(maybe) {
  if (maybe[0] === "Just") {
    const a = maybe[1];
    return (b, c) => [a, b, c];
  }
  if (maybe[0] === "Nothing") {
    return (b, c) => [1, b, c];
  }
  throw new Error("Pattern match error");
}
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
  A,
  B,
  C,
  Just,
  ManyFields,
  Nothing,
  complex_matched_expresion,
  effect_arms,
  function_arms,
  id,
  is_just,
  many_fields_to_array,
  mk_five,
  nested_pattern,
  to_string,
  very_function_arms,
  with_default,
};
