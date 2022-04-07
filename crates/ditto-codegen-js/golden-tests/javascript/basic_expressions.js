function denied(a) {
  return undefined;
}
function always(a) {
  return b => a;
}
function uncurry(fn) {
  return (a, b) => fn(a)(b);
}
function curry(fn) {
  return a => b => fn(a, b);
}
function identity(a) {
  return a;
}
const fifthString = "A";
const notQuiteFive = 5.01;
const floatyFive = 5.0;
const five = 5;
const fives = [
  5,
  five,
  always(five)(floatyFive),
  uncurry(always)(five, true),
  (a => a)(5),
  uncurry(always)(five, fifthString),
  always(identity)(false)(five),
];
export { curry, fives };
