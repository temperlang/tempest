import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import kotlin.ranges.IntRange;
import lang.temper.lexer.MarkdownSupportKt;
import lang.temper.lexer.TaggedRange;

/**
 * Prints the Kotlin Temper lexer's markdown code blocks for each file, as the
 * reference for temper-literate. One line per range:
 *
 *     tags <TAB> beginStart <TAB> beginEnd <TAB> endStart <TAB> endEnd
 *
 * Offsets are UTF-8 byte offsets with exclusive ends. Tags are "-" for a
 * paragraph (null), "" for an untagged code block, else space-separated.
 * Each file starts with a line "# <path>".
 *
 * Build and run with the jars of an installed Temper on the classpath; see
 * compat/check-literate.sh.
 */
public class MarkdownBlocks {
    public static void main(String[] args) throws Exception {
        for (String arg : args) {
            String text = Files.readString(Path.of(arg), StandardCharsets.UTF_8);
            System.out.println("# " + arg);
            List<TaggedRange> ranges = MarkdownSupportKt.findMarkdownCodeBlocks(text);
            for (TaggedRange range : ranges) {
                List<String> tags = range.getTags();
                String tagText = tags == null ? "-" : String.join(" ", tags);
                System.out.println(String.join("\t",
                    tagText,
                    bytes(text, range.getBegin().getFirst()),
                    bytes(text, range.getBegin().getLast() + 1),
                    bytes(text, range.getEnd().getFirst()),
                    bytes(text, range.getEnd().getLast() + 1)));
            }
        }
    }

    /** UTF-8 byte offset of a UTF-16 index. */
    private static String bytes(String text, int utf16Index) {
        int clamped = Math.max(0, Math.min(utf16Index, text.length()));
        return Integer.toString(text.substring(0, clamped).getBytes(StandardCharsets.UTF_8).length);
    }
}
