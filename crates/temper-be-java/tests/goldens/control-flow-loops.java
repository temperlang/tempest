package loops;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class LoopsGlobal {
    private LoopsGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("loops"));
        {
            int i = 0;
            while (i < 5) {
                if (i == 1 || i == 3) {
                    i = i + 1;
                    continue;
                }
                console.log(Integer.toString(i));
                i = i + 1;
            }
        }
        {
            int i2 = 0;
            while (i2 < 5) {
                if (i2 == 3) {
                    break;
                }
                console.log(Integer.toString(i2));
                i2 = i2 + 1;
            }
        }
        int n = 0;
        {
            int i3 = 0;
            outer: while (i3 < 4) {
                String str = "row " + i3 + " =";
                int j = 0;
                while (true) {
                    String t3 = str + " ";
                    int t = n;
                    n = t + 1;
                    str = t3 + t;
                    if (i3 <= j) {
                        console.log(str);
                        i3 = i3 + 1;
                        continue outer;
                    }
                    j = j + 1;
                }
            }
        }
        int prime = 1;
        {
            int i4 = 0;
            outer2: while (i4 < 25) {
                {
                    int j2 = 0;
                    outer3: while (j2 < 25) {
                        {
                            int k = 0;
                            while (k < 25) {
                                prime = prime + 1;
                                if (prime != 2 && prime % 2 == 0) {
                                    i4 = i4 + 1;
                                    continue outer2;
                                }
                                if (prime != 3 && prime % 3 == 0) {
                                    j2 = j2 + 1;
                                    continue outer3;
                                }
                                if (prime != 5 && prime % 5 == 0) {
                                    k = k + 1;
                                    continue;
                                }
                                if (prime != 7 && prime % 7 == 0) {
                                    k = k + 1;
                                    continue;
                                }
                                console.log(Integer.toString(prime));
                                k = k + 1;
                            }
                        }
                        j2 = j2 + 1;
                    }
                }
                i4 = i4 + 1;
            }
        }
        int val = 0;
        {
            int i5 = 0;
            outer4: while (i5 < 10) {
                String str2 = "row " + i5 + " =";
                int j3 = 0;
                while (true) {
                    String t4 = str2 + " ";
                    int t2 = val;
                    val = t2 + 1;
                    str2 = t4 + t2;
                    if (i5 == 4) {
                        console.log("break");
                        break outer4;
                    }
                    if (i5 <= j3) {
                        console.log(str2);
                        i5 = i5 + 1;
                        continue outer4;
                    }
                    j3 = j3 + 1;
                }
            }
        }
    }
}
