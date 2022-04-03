export function h(name, attrs) {
  return;
}

/**
 * @template A
 * @template B
 * @param {A[]} array
 * @param {(element: A) => B} f
 * @returns {B[]}
 */
export function arrayMapImpl(array, f) {
  return array.map(f);
}
