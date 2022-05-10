function denied(a) {
  return;
}
function select(c, x, y) {
  return c ? x : y;
}
function always(a) {
  return _b => a;
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
  ((a, _b) => a)(5, undefined),
  uncurry(always)(five, fifth_string),
  always(identity)(false)(five),
];
export { curry, fives };
