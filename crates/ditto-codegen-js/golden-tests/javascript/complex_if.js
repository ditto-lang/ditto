const result = (() => {
  if (true) {
    return (() => {
      if (true) {
        return (() => {
          if (true) {
            return 0;
          } else {
            return 1;
          }
        })();
      } else {
        return 2;
      }
    })();
  } else {
    return 3;
  }
})();
export { result };
