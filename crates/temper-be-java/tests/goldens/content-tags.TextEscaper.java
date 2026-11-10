package content_tags;

public final class TextEscaper implements Escaper {
    public static final TextEscaper instance = new TextEscaper();

    public TextEscaper() {}

    public String applyString(String x) {
        return "[" + x + "]";
    }
}
