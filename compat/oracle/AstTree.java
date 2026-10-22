import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import kotlin.Pair;
import lang.temper.ast.CstPart;
import lang.temper.ast.FlattenKt;
import lang.temper.astbuild.BuildTreeKt;
import lang.temper.astbuild.StoredCommentTokens;
import lang.temper.common.AtomicCounter;
import lang.temper.common.Log;
import lang.temper.cst.ConcreteSyntaxTree;
import lang.temper.cst.CstComment;
import lang.temper.cst.CstInner;
import lang.temper.frontend.parse.ParseStageKt;
import lang.temper.lexer.Genre;
import lang.temper.lexer.LanguageConfig;
import lang.temper.lexer.Lexer;
import lang.temper.lexer.MarkdownLanguageConfig;
import lang.temper.lexer.StandaloneLanguageConfig;
import lang.temper.log.AbstractMaxLevelLogSink;
import lang.temper.log.ConfigurationKey;
import lang.temper.log.LogSink;
import lang.temper.log.MessageTemplateI;
import lang.temper.log.Position;
import lang.temper.log.SharedLocationContext;
import lang.temper.log.UnknownCodeLocation;
import lang.temper.name.BuiltinName;
import lang.temper.name.CoreCodeLocation;
import lang.temper.name.ModuleLocation;
import lang.temper.name.NamingContext;
import lang.temper.name.ParsedName;
import lang.temper.name.Symbol;
import lang.temper.name.TemperName;
import lang.temper.parser.ParseKt;
import lang.temper.parser.TokenSourceAdapterBuilder;
import lang.temper.value.BlockTree;
import lang.temper.value.CallTree;
import lang.temper.value.DeclTree;
import lang.temper.value.DependencyCategory;
import lang.temper.value.DocumentContext;
import lang.temper.value.EscTree;
import lang.temper.value.FunTree;
import lang.temper.value.LeftNameLeaf;
import lang.temper.value.RightNameLeaf;
import lang.temper.value.Tree;
import lang.temper.value.Value;
import lang.temper.value.ValueLeaf;

/**
 * Prints the tree Kotlin Temper's grammar step (lang.temper.astbuild.buildTree)
 * makes for each file, normalized for comparison with temper-syntax's
 * (crates/temper-syntax/src/bin/ast-tree.rs). As in ParseStage, the parse
 * tree is first split at `;;;` into code and JSON metadata; the code is
 * built from the Root production and the metadata, if any, from Json.
 *
 * Each file starts with "# <path>", then one line per tree node in
 * pre-order, then the grammar step's diagnostics (not the parser's):
 *
 *     N <TAB> depth <TAB> Block|Call|Decl|Esc|Fun <TAB> start <TAB> end
 *     L <TAB> depth <TAB> LeftName|RightName|BuiltinName <TAB> start <TAB> end <TAB> name
 *     V <TAB> depth <TAB> start <TAB> end <TAB> type <TAB> content...
 *     X <TAB> depth <TAB> start <TAB> end
 *     M                                  (metadata follows)
 *     E <TAB> level <TAB> template <TAB> values... <TAB> start <TAB> end
 *
 * X is an error tree, a call to `error`, whose contents (the unparsable
 * parts as strings) aren't printed. Values print as their type and content:
 * Float64 as the hex of its bits, functions by name, lists as a count and
 * escaped strings. Offsets are UTF-8 byte offsets, ends at least starts
 * (Kotlin gives empty trees an end before their start). If Kotlin throws, the
 * file's output ends with a line starting "Crash".
 *
 * With "--bench ROUNDS" first, it instead reads all files, and per round
 * lexes, parses and builds trees for them all, printing milliseconds per
 * round.
 */
public class AstTree {
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
            try {
                Pair<Tree, Tree> trees = build(text, language(arg), sink);
                tree(trees.getFirst(), 0, text, bytes, out);
                if (trees.getSecond() != null) {
                    out.append("M\n");
                    tree(trees.getSecond(), 0, text, bytes, out);
                }
            } catch (RuntimeException | StackOverflowError e) {
                out.append("Crash\t").append(e.getClass().getSimpleName()).append('\n');
                System.err.println(arg + ":");
                e.printStackTrace();
            }
            for (Entry entry : sink.entries) {
                out.append("E\t").append(entry.level.toString()).append('\t').append(name(entry.template));
                for (Object value : entry.values) {
                    out.append('\t');
                    escape(String.valueOf(value), out);
                }
                appendSpan(entry.pos, text, bytes, out);
                out.append('\n');
            }
            stdout.print(out);
            out.setLength(0);
        }
        stdout.flush();
    }

    /** The code's tree and the metadata's, if any. */
    private static Pair<Tree, Tree> build(String text, LanguageConfig language, LogSink sink) {
        Lexer lexer = new Lexer(UnknownCodeLocation.INSTANCE, LogSink.Companion.getDevNull(), text, 0, language, false, false);
        List<CstComment> comments = new ArrayList<>();
        CstInner root = ParseKt.parse(lexer, LogSink.Companion.getDevNull(), comments, new TokenSourceAdapterBuilder());
        Pair<ConcreteSyntaxTree, ConcreteSyntaxTree> split = ParseStageKt.splitCodeFromMetadata(root, sink);
        List<CstPart> parts = FlattenKt.flatten(split.getFirst());
        Tree code = BuildTreeKt.buildTree(parts, StoredCommentTokens.Companion.invoke(comments), sink, new Context(), "Root");
        Tree metadata = null;
        if (split.getSecond() != null) {
            metadata = BuildTreeKt.buildTree(
                FlattenKt.flatten(split.getSecond()), StoredCommentTokens.Companion.getEmpty(), sink, new Context(), "Json");
        }
        return new Pair<>(code, metadata);
    }

    private static void tree(Tree node, int depth, String text, int[] bytes, StringBuilder out) {
        if (node instanceof ValueLeaf leaf) {
            out.append("V\t").append(depth);
            appendSpan(node.getPos(), text, bytes, out);
            out.append('\t');
            value(leaf.getContent(), out);
            out.append('\n');
        } else if (node instanceof LeftNameLeaf || node instanceof RightNameLeaf) {
            TemperName name = (TemperName) node.getContent();
            String kind = node instanceof LeftNameLeaf ? "LeftName" : "RightName";
            String nameText;
            if (name instanceof BuiltinName builtin) {
                kind = "BuiltinName";
                nameText = builtin.getBuiltinKey();
            } else if (name instanceof ParsedName parsed) {
                nameText = parsed.getNameText();
            } else {
                nameText = "?" + name;
            }
            out.append("L\t").append(depth).append('\t').append(kind);
            appendSpan(node.getPos(), text, bytes, out);
            out.append('\t');
            escape(nameText, out);
            out.append('\n');
        } else if (isError(node)) {
            out.append("X\t").append(depth);
            appendSpan(node.getPos(), text, bytes, out);
            out.append('\n');
        } else {
            String kind = node instanceof BlockTree ? "Block"
                : node instanceof CallTree ? "Call"
                : node instanceof DeclTree ? "Decl"
                : node instanceof EscTree ? "Esc"
                : node instanceof FunTree ? "Fun"
                : node.getClass().getSimpleName();
            out.append("N\t").append(depth).append('\t').append(kind);
            appendSpan(node.getPos(), text, bytes, out);
            out.append('\n');
            for (int i = 0; i < node.getSize(); i++) {
                tree(node.child(i), depth + 1, text, bytes, out);
            }
        }
    }

    private static boolean isError(Tree node) {
        return node instanceof CallTree && node.getSize() > 0 && node.child(0) instanceof ValueLeaf callee
            && callee.getContent().toString().equals("error: Function");
    }

    private static void value(Value<?> value, StringBuilder out) {
        String type = value.getTypeTag().toString();
        Object state = value.getStateVector();
        out.append(type);
        switch (type) {
            case "Int32", "Int64", "Boolean" -> out.append('\t').append(state);
            case "Float64" -> out.append('\t').append(Long.toHexString(Double.doubleToRawLongBits((Double) state)));
            case "String" -> {
                out.append('\t');
                escape((String) state, out);
            }
            case "Symbol" -> {
                out.append('\t');
                escape(((Symbol) state).getText(), out);
            }
            case "List" -> {
                List<?> items = (List<?>) state;
                out.append('\t').append(items.size());
                for (Object item : items) {
                    out.append('\t');
                    escape(String.valueOf(((Value<?>) item).getStateVector()), out);
                }
            }
            case "Null", "Void" -> {}
            default -> {
                String shown = value.toString();
                String suffix = ": " + type;
                if (shown.endsWith(suffix)) {
                    shown = shown.substring(0, shown.length() - suffix.length());
                }
                out.append('\t');
                escape(shown, out);
            }
        }
    }

    /** A document context with what buildTree needs. */
    private static final class Context extends NamingContext implements DocumentContext, ConfigurationKey {
        Context() {
            super(new AtomicCounter());
        }

        @Override
        public ModuleLocation getLoc() {
            return CoreCodeLocation.INSTANCE;
        }

        @Override
        public SharedLocationContext getSharedLocationContext() {
            return null;
        }

        @Override
        public AtomicCounter getDefinitionMutationCounter() {
            return new AtomicCounter();
        }

        @Override
        public NamingContext getNamingContext() {
            return this;
        }

        @Override
        public Genre getGenre() {
            return Genre.Library;
        }

        @Override
        public DependencyCategory getDependencyCategory() {
            return DependencyCategory.Production;
        }

        @Override
        public ConfigurationKey getConfigurationKey() {
            return this;
        }
    }

    private record Entry(Log.Level level, MessageTemplateI template, Position pos, List<?> values) {}

    private static final class Sink extends AbstractMaxLevelLogSink {
        final List<Entry> entries = new ArrayList<>();

        @Override
        protected void doLog(Log.Level level, MessageTemplateI template, Position pos, List<?> values, boolean fyi) {
            entries.add(new Entry(level, template, pos, values));
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
                try {
                    count += build(texts[i], language(paths[i]), LogSink.Companion.getDevNull()).getFirst().getSize();
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
        int start = bytes[clamp(pos.getLeft(), text)];
        int end = bytes[clamp(pos.getRight(), text)];
        out.append('\t').append(start).append('\t').append(Math.max(start, end));
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
