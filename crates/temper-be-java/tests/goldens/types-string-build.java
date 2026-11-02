package build;

import java.util.logging.Logger;
import temper.core.Core;
import temper.core.Core.Console;

public final class BuildGlobal {
    private BuildGlobal() {}

    static {
        Console console = Core.getConsole(Logger.getLogger("build"));
        StringBuilder builder = new StringBuilder();
        builder.append("Hi ");
        Core.stringBuilderAppendCodePoint(builder, 127759);
        String source = "Wow!! Neato!";
        int bangIndex = Core.stringStep(source, 0, 3);
        int spaceIndex = Core.stringStep(source, bangIndex, 2);
        Core.stringBuilderAppendBetween(builder, source, bangIndex, spaceIndex);
        String built = builder.toString();
        builder.append("\n");
        builder.append(built);
        console.log(builder.toString());
        try {
            Core.stringBuilderAppendCodePoint(builder, 1114112);
        } catch (RuntimeException ignored) {
            console.log("High code blocked.");
        }
        try {
            Core.stringBuilderAppendCodePoint(builder, 55296);
        } catch (RuntimeException ignored2) {
            console.log("Surrogate blocked.");
        }
        console.log(builder.toString());
        builder.setLength(0);
        builder.append("Back to square zero.");
        console.log(builder.toString());
        StringBuilderOwner owner = new StringBuilderOwner(new StringBuilder());
        owner.builder.append("Hi from inside.");
        console.log(owner.builder.toString());
    }
}
