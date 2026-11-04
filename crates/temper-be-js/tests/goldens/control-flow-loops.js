import { modIntInt } from "@temperlang/core";

{
  let i = 0;
  while (i < 5) {
    if (i === 1 || i === 3) {
      {
        i = (i + 1) | 0;
        continue;
      }
    }
    console.log(i.toString());
    i = (i + 1) | 0;
  }
}
{
  let i2 = 0;
  while (i2 < 5) {
    if (i2 === 3) {
      break;
    }
    console.log(i2.toString());
    i2 = (i2 + 1) | 0;
  }
}
let n = 0;
{
  {
    let i3 = 0;
    outer: while (i3 < 4) {
      let str = `row ${i3} =`;
      let j = 0;
      while (true) {
        /** @type {number} */
        let t$0;
        {
          const t = n;
          n = (t + 1) | 0;
          t$0 = t;
        }
        str = `${str} ${t$0}`;
        if (i3 <= j) {
          console.log(str);
          {
            i3 = (i3 + 1) | 0;
            continue outer;
          }
        }
        j = (j + 1) | 0;
      }
    }
  }
}
let prime = 1;
{
  {
    let i4 = 0;
    outer2: while (i4 < 25) {
      {
        {
          let j2 = 0;
          outer3: while (j2 < 25) {
            {
              {
                let k = 0;
                while (k < 25) {
                  prime = (prime + 1) | 0;
                  if (prime !== 2 && modIntInt(prime, 2) === 0) {
                    {
                      i4 = (i4 + 1) | 0;
                      continue outer2;
                    }
                  }
                  if (prime !== 3 && modIntInt(prime, 3) === 0) {
                    {
                      j2 = (j2 + 1) | 0;
                      continue outer3;
                    }
                  }
                  if (prime !== 5 && modIntInt(prime, 5) === 0) {
                    {
                      k = (k + 1) | 0;
                      continue;
                    }
                  }
                  if (prime !== 7 && modIntInt(prime, 7) === 0) {
                    {
                      k = (k + 1) | 0;
                      continue;
                    }
                  }
                  console.log(prime.toString());
                  k = (k + 1) | 0;
                }
              }
            }
            j2 = (j2 + 1) | 0;
          }
        }
      }
      i4 = (i4 + 1) | 0;
    }
  }
}
let val = 0;
{
  {
    let i5 = 0;
    outer4: while (i5 < 10) {
      let str2 = `row ${i5} =`;
      let j3 = 0;
      while (true) {
        /** @type {number} */
        let t$1;
        {
          const t2 = val;
          val = (t2 + 1) | 0;
          t$1 = t2;
        }
        str2 = `${str2} ${t$1}`;
        if (i5 === 4) {
          console.log("break");
          break outer4;
        }
        if (i5 <= j3) {
          console.log(str2);
          {
            i5 = (i5 + 1) | 0;
            continue outer4;
          }
        }
        j3 = (j3 + 1) | 0;
      }
    }
  }
}
