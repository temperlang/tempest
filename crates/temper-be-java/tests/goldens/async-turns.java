package async_turns;

import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;
import java.util.function.IntConsumer;
import java.util.function.Supplier;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Generator;
import temper.core.Generator.DoneResult;
import temper.core.Generator.Result;
import temper.core.Generator.ValueResult;
import temper.core.Nullable;

public final class AsyncTurnsGlobal {
    private AsyncTurnsGlobal() {}

    static final Console console;
    static final Counter c;
    static final CompletableFuture<Integer> gateA;
    static final CompletableFuture<Integer> gateB;

    public static CompletableFuture<Integer> later(int n) {
        CompletableFuture<Integer> pb = new CompletableFuture<>();
        Supplier<Generator<Optional<? super Object>>> fn = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                int caseIndexLocal = caseIndex.value;
                caseIndex.value = -1;
                if (caseIndexLocal == 0) {
                    pb.complete(n * 2);
                    return DoneResult.get();
                } else {
                    return DoneResult.get();
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        Core.runAsync(fn);
        return pb;
    }

    static Generator<Optional<? super Object>> generated(Supplier<Generator<Optional<? super Object>>> body) {
        return body.get();
    }

    public static Generator<Optional<? super Object>> ticks(int n, IntConsumer each) {
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
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                while (true) {
                    int caseIndexLocal = caseIndex.value;
                    caseIndex.value = -1;
                    if (caseIndexLocal == 0) {
                        i.value = 0;
                        caseIndex.value = 1;
                    } else if (caseIndexLocal == 1) {
                        if (i.value < n) {
                            caseIndex.value = 2;
                        } else {
                            caseIndex.value = 3;
                        }
                    } else if (caseIndexLocal == 2) {
                        each.accept(i.value);
                        caseIndex.value = 4;
                        return new ValueResult<>(Optional.empty());
                    } else if (caseIndexLocal == 4) {
                        i.value = i.value + 1;
                        caseIndex.value = 1;
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        return generated(fn);
    }

    static {
        console = Core.getConsole(Logger.getLogger("async_turns"));
        Generator<Optional<? super Object>> t = ticks(2, i -> console.log("tick " + i));
        t.get();
        t.get();
        t.get();
        c = new Counter();
        gateA = new CompletableFuture<>();
        gateB = new CompletableFuture<>();
        Supplier<Generator<Optional<? super Object>>> fn2 = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            class KCell {
                int value;
            }
            final KCell k = new KCell();
            k.value = 0;
            class AwaitedCell {
                @Nullable CompletableFuture<Integer> value;
            }
            final AwaitedCell awaited = new AwaitedCell();
            awaited.value = null;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                while (true) {
                    int caseIndexLocal = caseIndex.value;
                    caseIndex.value = -1;
                    if (caseIndexLocal == 0) {
                        c.bump("A", 10000);
                        caseIndex.value = 3;
                    } else if (caseIndexLocal == 1) {
                        k.value = -1;
                        caseIndex.value = 2;
                    } else if (caseIndexLocal == 2) {
                        c.bump("A" + k.value, 10000);
                        return DoneResult.get();
                    } else if (caseIndexLocal == 3) {
                        try {
                            awaited.value = gateA;
                            caseIndex.value = 4;
                            Core.awakeUpon(awaited.value, generator);
                            return new ValueResult<>(Optional.empty());
                        } catch (RuntimeException ignored) {
                            caseIndex.value = 1;
                        }
                    } else if (caseIndexLocal == 4) {
                        try {
                            k.value = Core.getPromiseResult(awaited.value);
                            caseIndex.value = 2;
                        } catch (RuntimeException ignored2) {
                            caseIndex.value = 1;
                        }
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        Core.runAsync(fn2);
        Supplier<Generator<Optional<? super Object>>> fn3 = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            class KCell {
                int value;
            }
            final KCell k = new KCell();
            k.value = 0;
            class AwaitedCell {
                @Nullable CompletableFuture<Integer> value;
            }
            final AwaitedCell awaited = new AwaitedCell();
            awaited.value = null;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                while (true) {
                    int caseIndexLocal = caseIndex.value;
                    caseIndex.value = -1;
                    if (caseIndexLocal == 0) {
                        c.bump("B", 10000);
                        caseIndex.value = 3;
                    } else if (caseIndexLocal == 1) {
                        k.value = -1;
                        caseIndex.value = 2;
                    } else if (caseIndexLocal == 2) {
                        c.bump("B" + k.value, 10000);
                        return DoneResult.get();
                    } else if (caseIndexLocal == 3) {
                        try {
                            awaited.value = gateB;
                            caseIndex.value = 4;
                            Core.awakeUpon(awaited.value, generator);
                            return new ValueResult<>(Optional.empty());
                        } catch (RuntimeException ignored3) {
                            caseIndex.value = 1;
                        }
                    } else if (caseIndexLocal == 4) {
                        try {
                            k.value = Core.getPromiseResult(awaited.value);
                            caseIndex.value = 2;
                        } catch (RuntimeException ignored4) {
                            caseIndex.value = 1;
                        }
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        Core.runAsync(fn3);
        Supplier<Generator<Optional<? super Object>>> fn4 = () -> {
            class CaseIndexCell {
                int value;
            }
            final CaseIndexCell caseIndex = new CaseIndexCell();
            caseIndex.value = 0;
            Function<Generator<Optional<? super Object>>, Result<Optional<? super Object>>> convertedCoroutine = generator -> {
                int caseIndexLocal = caseIndex.value;
                caseIndex.value = -1;
                if (caseIndexLocal == 0) {
                    console.log("opener: calling complete");
                    gateA.complete(1);
                    gateB.complete(2);
                    console.log("opener: complete returned");
                    return DoneResult.get();
                } else {
                    return DoneResult.get();
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        Core.runAsync(fn4);
    }
}
