import {
  denseBitVectorConstructor,
  denseBitVectorGet,
  denseBitVectorSet,
} from "@temperlang/core";

const v = denseBitVectorConstructor(16);
console.log(`v[2]=${denseBitVectorGet(v, 2)}`);
console.log(`v[3]=${denseBitVectorGet(v, 3)}`);
console.log("v[3] <- true");
denseBitVectorSet(v, 3, true);
console.log(`v[2]=${denseBitVectorGet(v, 2)}`);
console.log(`v[3]=${denseBitVectorGet(v, 3)}`);
console.log("v[3] <- false");
denseBitVectorSet(v, 3, false);
console.log(`v[2]=${denseBitVectorGet(v, 2)}`);
console.log(`v[3]=${denseBitVectorGet(v, 3)}`);
