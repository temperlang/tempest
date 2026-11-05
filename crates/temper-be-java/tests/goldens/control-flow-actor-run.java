package actor_run;

import java.util.Optional;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;
import temper.core.Generator;
import temper.core.Generator.DoneResult;
import temper.core.Generator.Result;
import temper.core.Generator.ValueResult;

public final class ActorRunGlobal {
    private ActorRunGlobal() {}

    static final Console console;

    static void f(Supplier<Generator<Optional<? super Object>>> body) {
        Generator<Optional<? super Object>> generator = body.get();
        console.log("f0");
        generator.get();
        console.log("f1");
        generator.get();
        console.log("f2");
        generator.get();
        console.log("f3");
    }

    static {
        console = Core.getConsole(Logger.getLogger("actor_run"));
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
                    if (caseIndexLocal == 0) {
                        i.value = 0;
                        caseIndex.value = 1;
                    } else if (caseIndexLocal == 1) {
                        if (true) {
                            caseIndex.value = 2;
                        } else {
                            caseIndex.value = 3;
                        }
                    } else if (caseIndexLocal == 2) {
                        console.log("generated " + i.value);
                        i.value = i.value + 1;
                        caseIndex.value = 4;
                        return new ValueResult<>(Optional.empty());
                    } else if (caseIndexLocal == 3) {
                        caseIndex.value = -1;
                        return DoneResult.get();
                    } else if (caseIndexLocal == 4) {
                        caseIndex.value = 1;
                    } else {
                        return DoneResult.get();
                    }
                }
            };
            return Core.safeAdaptGeneratorFn(convertedCoroutine::apply);
        };
        f(fn);
    }
}
