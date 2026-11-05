package functions;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.function.Function;
import java.util.function.IntSupplier;
import java.util.function.IntUnaryOperator;
import java.util.function.Supplier;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Generator;
import temper.core.Generator.DoneResult;
import temper.core.Generator.Result;
import temper.core.Generator.ValueResult;
import temper.core.Nullable;

public final class FunctionsGlobal {
    private FunctionsGlobal() {}

    static final Console console;

    static IntSupplier makeCounter(int start) {
        class CountCell {
            int value;
        }
        final CountCell count = new CountCell();
        count.value = start;
        IntSupplier fn = () -> {
            count.value = count.value + 1;
            return count.value;
        };
        return fn;
    }

    static IntUnaryOperator twice(IntUnaryOperator f) {
        return x -> f.applyAsInt(f.applyAsInt(x));
    }

    static int applyTo(IntUnaryOperator f, int x) {
        return f.applyAsInt(x);
    }

    static int addOne(int x) {
        return x + 1;
    }

    static void drive(int steps, Supplier<Generator<Optional<? super Object>>> body) {
        Generator<Optional<? super Object>> g = body.get();
        {
            int s = 0;
            while (s < steps) {
                g.get();
                s = s + 1;
            }
        }
    }

    static {
        console = Core.getConsole(Logger.getLogger("functions"));
        IntSupplier counter = makeCounter(10);
        counter.getAsInt();
        console.log("count " + counter.getAsInt());
        IntSupplier other = makeCounter(0);
        console.log("other " + other.getAsInt() + " count " + counter.getAsInt());
        IntUnaryOperator addTwo = twice(FunctionsGlobal::addOne);
        console.log("twice " + addTwo.applyAsInt(5) + " " + applyTo(FunctionsGlobal::addOne, 1));
        console.log("triple " + applyTo(Scaler::triple, 4) + " " + twice(Scaler::triple).applyAsInt(2));
        List<Integer> ns = List.of(5, 2, 8, 1, 4);
        List<Integer> doubled = Core.listMapIntToInt(ns, n -> n * 2);
        List<Integer> big = Core.listFilterInt(doubled, n -> n > 4);
        console.log(Core.listJoinInt(big, ", ", n -> Integer.toString(n)));
        List<Integer> order = Core.listSortedInt(ns, (a, b) -> a - b);
        console.log(Core.listJoinInt(order, " ", n -> Integer.toString(n)));
        int total = 0;
        {
            int length = ns.size();
            int i2 = 0;
            while (i2 < length) {
                int n = Core.listGet(ns, i2);
                i2 = i2 + 1;
                total = total + n;
            }
        }
        console.log("total " + total);
        Button button = new Button("hits", makeCounter(0));
        button.press();
        console.log(button.press());
        List<IntSupplier> makers = new ArrayList<>();
        {
            int i = 0;
            while (i < 3) {
                int k = i * 10;
                makers.add(() -> k);
                i = i + 1;
            }
        }
        console.log(Core.listJoinObj(makers, " ", c -> Integer.toString(c.getAsInt())));
        Supplier<Generator<Optional<? super Object>>> fn8 = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            class FsCell {
                @Nullable List<IntSupplier> value;
            }
            final FsCell fs = new FsCell();
            fs.value = null;
            class ICell {
                int value;
            }
            final ICell i = new ICell();
            i.value = 0;
            class JCell {
                int value;
            }
            final JCell j = new JCell();
            j.value = 0;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                while (true) {
                    int caseIndexLocal = caseIndex.value;
                    caseIndex.value = -1;
                    if (caseIndexLocal == 0) {
                        fs.value = new ArrayList<>();
                        i.value = 0;
                        caseIndex.value = 1;
                    } else if (caseIndexLocal == 1) {
                        if (i.value < 3) {
                            caseIndex.value = 2;
                        } else {
                            caseIndex.value = 3;
                        }
                    } else if (caseIndexLocal == 2) {
                        j.value = i.value * 10;
                        List<IntSupplier> t = fs.value;
                        IntSupplier t2;
                        {
                            int j2 = j.value;
                            t2 = () -> j2;
                        }
                        t.add(t2);
                        caseIndex.value = 4;
                        return new ValueResult<>(Optional.empty());
                    } else if (caseIndexLocal == 3) {
                        console.log(Core.listJoinObj(fs.value, " ", c -> Integer.toString(c.getAsInt())));
                        caseIndex.value = 5;
                        return new ValueResult<>(Optional.empty());
                    } else if (caseIndexLocal == 4) {
                        List<IntSupplier> t3 = fs.value;
                        IntSupplier t4;
                        {
                            int j2 = j.value;
                            t4 = () -> j2 + 1;
                        }
                        t3.add(t4);
                        i.value = i.value + 1;
                        caseIndex.value = 1;
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        drive(5, fn8);
    }
}
