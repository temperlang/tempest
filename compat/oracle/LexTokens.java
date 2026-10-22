import java.io.FileOutputStream;
import java.io.FileDescriptor;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import lang.temper.lexer.LanguageConfig;
import lang.temper.lexer.Lexer;
import lang.temper.lexer.MarkdownLanguageConfig;
import lang.temper.lexer.StandaloneLanguageConfig;
import lang.temper.lexer.TemperToken;
import lang.temper.log.LogSink;
import lang.temper.log.MessageTemplateI;
import lang.temper.log.Position;
import lang.temper.log.UnknownCodeLocation;

/**
 * Prints the tokens of Kotlin Temper's raw lexer for each file, as the
 * reference for temper-syntax. This is the lexer alone, before the parser's
 * token adapters (call joins, word pairing, semicolon insertion). One line
 * per token:
 *
 *     type <TAB> start <TAB> end <TAB> flags <TAB> error <TAB> text
 *
 * Offsets are UTF-8 byte offsets with exclusive ends. Flags are "s" for a
 * synthetic token and "b" for one that may bracket, or "-" for neither.
 * Error is the error message template name, or "-". Text is escaped so it
 * fits on one line: backslash, newline, CR, tab and other control characters.
 * If Kotlin's lexer throws, the file's tokens end with a line starting
 * "Crash". Each file starts with a line "# <path>". Files ending in ".md" lex as
 * literate markdown, others as standalone Temper.
 *
 * With "--bench ROUNDS" first, it instead reads all files, lexes them all
 * once per round, and prints the milliseconds per round, as
 * crates/temper-syntax/src/bin/lex-tokens.rs does. Run enough rounds that the
 * JVM warms up; compare the later rounds.
 */
public class LexTokens {
    public static void main(String[] args) throws Exception {
        // Write UTF-8 whatever the platform default.
        PrintStream stdout = new PrintStream(new FileOutputStream(FileDescriptor.out), false, StandardCharsets.UTF_8);
        if (args.length >= 2 && args[0].equals("--bench")) {
            bench(Integer.parseInt(args[1]), java.util.Arrays.copyOfRange(args, 2, args.length), stdout);
            return;
        }
        StringBuilder out = new StringBuilder();
        for (String arg : args) {
            String text = Files.readString(Path.of(arg), StandardCharsets.UTF_8);
            out.append("# ").append(arg).append('\n');
            Lexer lexer = new Lexer(
                UnknownCodeLocation.INSTANCE, LogSink.Companion.getDevNull(), text, 0, language(arg), false, false);
            int[] bytes = byteOffsets(text);
            while (true) {
                TemperToken token;
                try {
                    if (!lexer.hasNext()) {
                        break;
                    }
                    token = lexer.next();
                } catch (RuntimeException e) {
                    // Kotlin's lexer can throw on odd input. Report it as a
                    // line that never matches, so checks flag the input.
                    out.append("Crash\t-\t-\t-\t-\t").append(e.getClass().getSimpleName()).append('\n');
                    break;
                }
                Position pos = token.getPos();
                String flags = (token.getSynthetic() ? "s" : "") + (token.getMayBracket() ? "b" : "");
                MessageTemplateI error = token.getError();
                out.append(token.getTokenType().name()).append('\t')
                    .append(bytes[clamp(pos.getLeft(), text)]).append('\t')
                    .append(bytes[clamp(pos.getRight(), text)]).append('\t')
                    .append(flags.isEmpty() ? "-" : flags).append('\t')
                    .append(error == null ? "-" : errorName(error)).append('\t');
                escape(token.getTokenText(), out);
                out.append('\n');
            }
            stdout.print(out);
            out.setLength(0);
        }
        stdout.flush();
    }

    private static void bench(int rounds, String[] paths, PrintStream stdout) throws Exception {
        String[] texts = new String[paths.length];
        long bytes = 0;
        for (int i = 0; i < paths.length; i++) {
            texts[i] = Files.readString(Path.of(paths[i]), StandardCharsets.UTF_8);
            bytes += texts[i].getBytes(StandardCharsets.UTF_8).length;
        }
        stdout.printf("# %d files, %d bytes%n", paths.length, bytes);
        for (int round = 0; round < rounds; round++) {
            long begin = System.nanoTime();
            long count = 0;
            for (int i = 0; i < paths.length; i++) {
                Lexer lexer = new Lexer(
                    UnknownCodeLocation.INSTANCE, LogSink.Companion.getDevNull(), texts[i], 0,
                    language(paths[i]), false, false);
                while (lexer.hasNext()) {
                    lexer.next();
                    count++;
                }
            }
            double millis = (System.nanoTime() - begin) / 1e6;
            stdout.printf("%.3f\t%d%n", millis, count);
        }
        stdout.flush();
    }

    private static LanguageConfig language(String path) {
        return path.endsWith(".md") ? new MarkdownLanguageConfig() : StandaloneLanguageConfig.INSTANCE;
    }

    private static String errorName(MessageTemplateI error) {
        return error instanceof Enum<?> e ? e.name() : error.toString();
    }

    private static int clamp(int index, String text) {
        return Math.max(0, Math.min(index, text.length()));
    }

    /** UTF-8 byte offset for each UTF-16 index, including the end. */
    private static int[] byteOffsets(String text) {
        int[] bytes = new int[text.length() + 1];
        int n = 0;
        for (int i = 0; i < text.length(); i++) {
            bytes[i] = n;
            char c = text.charAt(i);
            if (c < 0x80) {
                n += 1;
            } else if (c < 0x800) {
                n += 2;
            } else if (Character.isHighSurrogate(c)) {
                n += 4;
                // The low surrogate shares the code point's offset end.
                if (i + 1 < text.length()) {
                    bytes[++i] = n;
                }
            } else {
                n += 3;
            }
        }
        bytes[text.length()] = n;
        return bytes;
    }

    private static void escape(String text, StringBuilder out) {
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            switch (c) {
                case '\\' -> out.append("\\\\");
                case '\n' -> out.append("\\n");
                case '\r' -> out.append("\\r");
                case '\t' -> out.append("\\t");
                default -> {
                    if (c < 0x20 || c == 0x7f) {
                        out.append(String.format("\\u{%x}", (int) c));
                    } else {
                        out.append(c);
                    }
                }
            }
        }
    }
}
