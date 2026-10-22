import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import lang.temper.common.Log;
import lang.temper.cst.ConcreteSyntaxTree;
import lang.temper.cst.CstComment;
import lang.temper.cst.CstInner;
import lang.temper.cst.CstLeaf;
import lang.temper.lexer.LanguageConfig;
import lang.temper.lexer.Lexer;
import lang.temper.lexer.MarkdownLanguageConfig;
import lang.temper.lexer.StandaloneLanguageConfig;
import lang.temper.lexer.TemperToken;
import lang.temper.log.AbstractMaxLevelLogSink;
import lang.temper.log.LogSink;
import lang.temper.log.MessageTemplateI;
import lang.temper.log.Position;
import lang.temper.log.UnknownCodeLocation;
import lang.temper.parser.ParseKt;
import lang.temper.parser.TokenSourceAdapterBuilder;

/**
 * Prints Kotlin Temper's parse tree for each file, normalized for comparison
 * with temper-syntax's parser (crates/temper-syntax/src/bin/parse-tree.rs).
 * This is the operator tree from lang.temper.parser.parse, after its token
 * adapters and post-processing, before the grammar turns it into calls.
 *
 * Each file starts with "# <path>", then one line per tree node in
 * pre-order, then the parser's errors and the comments it collected:
 *
 *     N <TAB> depth <TAB> operator
 *     T <TAB> depth <TAB> token type <TAB> start <TAB> end <TAB> flags <TAB> text
 *     E <TAB> template <TAB> values... <TAB> start <TAB> end
 *     C <TAB> comment type <TAB> start <TAB> end
 *
 * Offsets are UTF-8 byte offsets with exclusive ends. Flags are "s" for a
 * synthetic token or "-". Text is escaped as in LexTokens.java. Node spans
 * aren't printed: they're internal, and checked through error positions. If
 * Kotlin throws, the file's output ends with a line starting "Crash".
 *
 * With "--bench ROUNDS" first, it instead reads all files, lexes and parses
 * them all once per round, and prints the milliseconds per round.
 */
public class ParseTree {
    public static void main(String[] args) throws Exception {
        PrintStream stdout = new PrintStream(new FileOutputStream(FileDescriptor.out), false, StandardCharsets.UTF_8);
        if (args.length >= 2 && args[0].equals("--bench")) {
            bench(Integer.parseInt(args[1]), java.util.Arrays.copyOfRange(args, 2, args.length), stdout);
            return;
        }
        StringBuilder out = new StringBuilder();
        for (String arg : args) {
            String text = Files.readString(Path.of(arg), StandardCharsets.UTF_8);
            out.append("# ").append(arg).append('\n');
            int[] bytes = byteOffsets(text);
            Sink sink = new Sink();
            List<CstComment> comments = new ArrayList<>();
            try {
                Lexer lexer = new Lexer(
                    UnknownCodeLocation.INSTANCE, LogSink.Companion.getDevNull(), text, 0, language(arg), false, false);
                CstInner root = ParseKt.parse(lexer, sink, comments, new TokenSourceAdapterBuilder());
                tree(root, 0, text, bytes, out);
            } catch (RuntimeException | StackOverflowError e) {
                out.append("Crash\t").append(e.getClass().getSimpleName()).append('\n');
                System.err.println(arg + ":");
                e.printStackTrace();
            }
            for (Entry entry : sink.entries) {
                out.append("E\t").append(name(entry.template));
                for (Object value : entry.values) {
                    out.append('\t').append(value);
                }
                appendSpan(entry.pos, text, bytes, out);
                out.append('\n');
            }
            for (CstComment comment : comments) {
                out.append("C\t").append(comment.getType().name());
                appendSpan(comment.getPos(), text, bytes, out);
                out.append('\n');
            }
            stdout.print(out);
            out.setLength(0);
        }
        stdout.flush();
    }

    private static void tree(ConcreteSyntaxTree node, int depth, String text, int[] bytes, StringBuilder out) {
        if (node instanceof CstLeaf leaf) {
            TemperToken token = leaf.getTemperToken();
            out.append("T\t").append(depth).append('\t').append(token.getTokenType().name());
            appendSpan(token.getPos(), text, bytes, out);
            out.append('\t').append(token.getSynthetic() ? "s" : "-").append('\t');
            escape(token.getTokenText(), out);
            out.append('\n');
        } else {
            CstInner inner = (CstInner) node;
            out.append("N\t").append(depth).append('\t').append(inner.getOperator().name()).append('\n');
            for (ConcreteSyntaxTree child : inner.getOperands()) {
                tree(child, depth + 1, text, bytes, out);
            }
        }
    }

    private record Entry(MessageTemplateI template, Position pos, List<?> values) {}

    private static final class Sink extends AbstractMaxLevelLogSink {
        final List<Entry> entries = new ArrayList<>();

        @Override
        protected void doLog(Log.Level level, MessageTemplateI template, Position pos, List<?> values, boolean fyi) {
            entries.add(new Entry(template, pos, values));
        }
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
                try {
                    CstInner root = ParseKt.parse(lexer, LogSink.Companion.getDevNull(), null, new TokenSourceAdapterBuilder());
                    count += root.getOperands().size();
                } catch (RuntimeException e) {
                    count -= 1;
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

    private static String name(MessageTemplateI template) {
        return template instanceof Enum<?> e ? e.name() : template.toString();
    }

    private static void appendSpan(Position pos, String text, int[] bytes, StringBuilder out) {
        out.append('\t').append(bytes[clamp(pos.getLeft(), text)])
            .append('\t').append(bytes[clamp(pos.getRight(), text)]);
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
