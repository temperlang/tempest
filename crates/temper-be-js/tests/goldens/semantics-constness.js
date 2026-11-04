{
  let nci = 1;
  console.log(`nci = ${nci}`);
  /** @type {void} */
  let t$0;
  {
    nci = 2;
    t$0 = undefined;
  }
  t$0;
  console.log(`nci = ${nci}`);
  /** @type {number} */
  let nc;
  nc = 1;
  console.log(`nc = ${nc}`);
  /** @type {void} */
  let t$1;
  {
    nc = 2;
    t$1 = undefined;
  }
  t$1;
  console.log(`nc = ${nc}`);
  {
    let i = 0;
    while (i < 3) {
      /** @type {number} */
      let ncl;
      /** @type {void} */
      let t$2;
      {
        ncl = i;
        t$2 = undefined;
      }
      t$2;
      console.log(`ncl = ${ncl}`);
      i = (i + 1) | 0;
    }
  }
  const ci = 1;
  console.log(`ci = ${ci}`);
  console.log("failed to set ci");
  console.log(`ci = ${ci}`);
  /** @type {number} */
  let c;
  c = 1;
  console.log(`c = ${c}`);
  console.log("failed to set c");
  console.log(`c = ${c}`);
  {
    let i2 = 0;
    while (i2 < 3) {
      /** @type {number} */
      let cl;
      /** @type {void} */
      let t$3;
      {
        cl = i2;
        t$3 = undefined;
      }
      t$3;
      console.log(`cl = ${cl}`);
      i2 = (i2 + 1) | 0;
    }
  }
}
