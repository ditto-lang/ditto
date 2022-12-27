const empty_array = /** @type {readonly any[]} */ [];
function denied(a) {
  return;
}
function select(c, x, y) {
  return c ? x : y;
}
function always(a) {
  return _b$0 => a;
}
function uncurry(f) {
  return (a, b) => f(a)(b);
}
function curry(f) {
  return a => b => f(a, b);
}
function identity(a) {
  return a;
}
const fifth_string = "A";
const not_quite_five = 5.01;
const floaty_five = 5.0;
const five = 5;
const fives = [
  5,
  five,
  select(true, 5, 50),
  always(five)(floaty_five),
  uncurry(always)(five, true),
  ((a, _b$0) => a)(5, undefined),
  uncurry(always)(five, fifth_string),
  always(identity)(false)(five),
];
export { curry, fives };
