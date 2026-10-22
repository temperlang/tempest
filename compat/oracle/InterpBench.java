import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import lang.temper.common.Console;
import lang.temper.common.Log;
import lang.temper.common.TextOutput;
import lang.temper.frontend.Module;
import lang.temper.frontend.ModuleSource;
import lang.temper.lexer.Genre;
import lang.temper.lexer.StandaloneLanguageConfig;
import lang.temper.log.AbstractMaxLevelLogSink;
import lang.temper.log.FilePathKt;
import lang.temper.log.MessageTemplate;
import lang.temper.log.MessageTemplateI;
import lang.temper.log.Position;
import lang.temper.name.ModuleName;
import lang.temper.stage.Stage;

/**
 * Times Kotlin Temper's interpreter in one JVM, for compat/bench_interp.py.
 *
 *     InterpBench ROUNDS SMALL LARGE FILE...
 *
 * Each file is a program whose loop runs `N` times. Per round and file, it
 * builds and runs the program with `N` replaced by SMALL, then by LARGE, as
 * a fresh module taken through every stage to Run, as `temper repl` does
 * but without its step quota. It prints a line per run: the file, round,
 * SMALL and LARGE times in nanoseconds. The difference over LARGE - SMALL
 * is the cost of an iteration without compiling; the caller drops the
 * first rounds, while the JIT warms up. A run whose output isn't one line,
 * or that logs an error, stops the harness.
 */
public class InterpBench {
    public static void main(String[] args) throws Exception {
        int rounds = Integer.parseInt(args[0]);
        int small = Integer.parseInt(args[1]);
        int large = Integer.parseInt(args[2]);
        List<String> paths = List.of(args).subList(3, args.length);
        List<String> texts = new ArrayList<>();
        for (String path : paths) {
            texts.add(Files.readString(Path.of(path), StandardCharsets.UTF_8));
        }
        for (int round = 0; round < rounds; round++) {
            for (int i = 0; i < paths.size(); i++) {
                long a = time(texts.get(i).replace("N", Integer.toString(small)), paths.get(i));
                long b = time(texts.get(i).replace("N", Integer.toString(large)), paths.get(i));
                System.out.printf("%s\t%d\t%d\t%d%n", paths.get(i), round, a, b);
                System.out.flush();
            }
        }
    }

    /** Builds and runs a program, returning nanoseconds. */
    static long time(String text, String path) {
        Sink sink = new Sink();
        long begin = System.nanoTime();
        Module module = new Module(
                sink,
                new ModuleName(FilePathKt.dirPath("bench", "main"), 1, false),
                new Console(new Quiet(), Log.None.INSTANCE, null),
                () -> true,
                null,
                Genre.Library,
                false,
                null);
        // Only the file path, text and language; the rest default to null.
        module.deliverContent(new ModuleSource(
                null, null, text, null, null, FilePathKt.filePath("bench", "main", "bench.temper"), null,
                StandaloneLanguageConfig.INSTANCE, null, null));
        while (module.canAdvance()) {
            module.advance();
        }
        long elapsed = System.nanoTime() - begin;
        if (module.getStageCompleted() != Stage.Run || sink.errors > 0 || sink.output.size() != 1) {
            throw new IllegalStateException(path + ": " + module.getStageCompleted() + ", " + sink.errors
                    + " errors, output " + sink.output + ", " + module.getRunResult());
        }
        return elapsed;
    }

    private static final class Sink extends AbstractMaxLevelLogSink {
        final List<String> output = new ArrayList<>();
        int errors = 0;

        @Override
        protected void doLog(Log.Level level, MessageTemplateI template, Position pos, List<?> values, boolean fyi) {
            if (template == MessageTemplate.StandardOut) {
                output.add(values.toString());
            } else if (level.compareTo(Log.Error.INSTANCE) >= 0) {
                errors++;
            }
        }
    }

    private static final class Quiet extends TextOutput {
        @Override
        public boolean isTtyLike() {
            return false;
        }

        @Override
        public void emitLineChunk(CharSequence text) {}

        @Override
        public void flush() {}
    }
}
