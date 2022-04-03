const A = ["A"];
function Err($0) {
  return ["Err", $0];
}
function Internal($0, $1) {
  return ["Internal", $0, $1];
}
function Just($0) {
  return ["Just", $0];
}
const Nothing = ["Nothing"];
function Ok($0) {
  return ["Ok", $0];
}
const Private = ["Private"];
const justFive = Just(5);
const nothing = Nothing;
export { A, Err, Just, Nothing, Ok };
