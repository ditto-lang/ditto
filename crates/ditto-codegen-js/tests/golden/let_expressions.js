function Pair($0, $1) {
  return ["Pair", $0, $1];
}
function Pence($0) {
  return ["Pence", $0];
}
const many_lets = (() => {
  const one = 1;
  const two = 2;
  const three = 3;
  return [one, two, three];
})();
function pair_to_array(pair) {
  if (pair[0] === "Pair") {
    const snd = pair[2];
    const fst = pair[1];
    return [fst, snd];
  }
  throw new Error("Pattern match error");
}
function gbp_to_pence(gbp) {
  if (gbp[0] === "Pence") {
    const i = gbp[1];
    return i;
  }
  throw new Error("Pattern match error");
}
function mk_ints(i) {
  const five = 5;
  const six = 6;
  return [five, six, i, i, i];
}
const fives = (() => {
  const five = 5;
  return [five, five, five];
})();
export { Pair, Pence, fives, gbp_to_pence, many_lets, mk_ints, pair_to_array };
