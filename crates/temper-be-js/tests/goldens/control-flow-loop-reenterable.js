let i = 0;
while (i < 2) {
  const x = i;
  console.log(x.toString());
  i = (i + 1) | 0;
}
