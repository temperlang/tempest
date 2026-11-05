package await_in_loop;

import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Generator;
import temper.core.Generator.DoneResult;
import temper.core.Generator.Result;
import temper.core.Generator.ValueResult;
import temper.core.Nullable;

public final class AwaitInLoopGlobal {
    private AwaitInLoopGlobal() {}

    static final Console console;
    static final CompletableFuture<String> once;
    static final CompletableFuture<String> later;

    static {
        console = Core.getConsole(Logger.getLogger("await_in_loop"));
        once = new CompletableFuture<>();
        once.complete("same");
        later = new CompletableFuture<>();
        Supplier<Generator<Optional<? super Object>>> fn = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            class ICell {
                int value;
            }
            final ICell i = new ICell();
            i.value = 0;
            class SCell {
                String value;
            }
            final SCell s = new SCell();
            s.value = "";
            class JCell {
                int value;
            }
            final JCell j = new JCell();
            j.value = 0;
            class S2Cell {
                String value;
            }
            final S2Cell s2 = new S2Cell();
            s2.value = "";
            class TCell {
                String value;
            }
            final TCell t = new TCell();
            t.value = "";
            class AwaitedCell {
                @Nullable CompletableFuture<String> value;
            }
            final AwaitedCell awaited = new AwaitedCell();
            awaited.value = null;
            class Awaited2Cell {
                @Nullable CompletableFuture<String> value;
            }
            final Awaited2Cell awaited2 = new Awaited2Cell();
            awaited2.value = null;
            class Awaited3Cell {
                @Nullable CompletableFuture<String> value;
            }
            final Awaited3Cell awaited3 = new Awaited3Cell();
            awaited3.value = null;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                while (true) {
                    int caseIndexLocal = caseIndex.value;
                    caseIndex.value = -1;
                    if (caseIndexLocal == 0) {
                        i.value = 0;
                        caseIndex.value = 1;
                    } else if (caseIndexLocal == 1) {
                        if (i.value < 2) {
                            caseIndex.value = 2;
                        } else {
                            caseIndex.value = 3;
                        }
                    } else if (caseIndexLocal == 2) {
                        try {
                            awaited.value = once;
                            caseIndex.value = 6;
                            awaited.value.whenComplete((ignored$1, ignored$2) -> generator.get());
                            return new ValueResult<>(Optional.empty());
                        } catch (RuntimeException ignored) {
                            caseIndex.value = 4;
                        }
                    } else if (caseIndexLocal == 3) {
                        j.value = 0;
                        caseIndex.value = 7;
                    } else if (caseIndexLocal == 4) {
                        s.value = "?";
                        caseIndex.value = 5;
                    } else if (caseIndexLocal == 5) {
                        console.log(s.value + " " + i.value);
                        i.value = i.value + 1;
                        caseIndex.value = 1;
                    } else if (caseIndexLocal == 6) {
                        try {
                            s.value = Core.getPromiseResult(awaited.value);
                            caseIndex.value = 5;
                        } catch (RuntimeException ignored2) {
                            caseIndex.value = 4;
                        }
                    } else if (caseIndexLocal == 7) {
                        if (j.value < 2) {
                            caseIndex.value = 8;
                        } else {
                            caseIndex.value = 9;
                        }
                    } else if (caseIndexLocal == 8) {
                        try {
                            awaited2.value = later;
                            caseIndex.value = 12;
                            awaited2.value.whenComplete((ignored$1, ignored$2) -> generator.get());
                            return new ValueResult<>(Optional.empty());
                        } catch (RuntimeException ignored3) {
                            caseIndex.value = 10;
                        }
                    } else if (caseIndexLocal == 9) {
                        try {
                            awaited3.value = once;
                            caseIndex.value = 15;
                            awaited3.value.whenComplete((ignored$1, ignored$2) -> generator.get());
                            return new ValueResult<>(Optional.empty());
                        } catch (RuntimeException ignored4) {
                            caseIndex.value = 13;
                        }
                    } else if (caseIndexLocal == 10) {
                        s2.value = "?";
                        caseIndex.value = 11;
                    } else if (caseIndexLocal == 11) {
                        console.log(s2.value + " " + j.value);
                        j.value = j.value + 1;
                        caseIndex.value = 7;
                    } else if (caseIndexLocal == 12) {
                        try {
                            s2.value = Core.getPromiseResult(awaited2.value);
                            caseIndex.value = 11;
                        } catch (RuntimeException ignored5) {
                            caseIndex.value = 10;
                        }
                    } else if (caseIndexLocal == 13) {
                        t.value = "?";
                        caseIndex.value = 14;
                    } else if (caseIndexLocal == 14) {
                        console.log(t.value + " after the loops");
                        return DoneResult.get();
                    } else if (caseIndexLocal == 15) {
                        try {
                            t.value = Core.getPromiseResult(awaited3.value);
                            caseIndex.value = 14;
                        } catch (RuntimeException ignored6) {
                            caseIndex.value = 13;
                        }
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        Core.runAsync(fn);
        later.complete("later");
    }
}
