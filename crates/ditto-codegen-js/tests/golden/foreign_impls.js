import {
  array_map_impl as foreign$array_map_impl,
  h as foreign$h,
} from "./foreign.js";
function Attr($0, $1) {
  return ["Attr", $0, $1];
}
const array_map = foreign$array_map_impl;
function span(attrs) {
  return foreign$h("span", attrs);
}
function div(attrs) {
  return foreign$h("div", attrs);
}
export { Attr, array_map, div, span };
