package content_tags;

public final class QuoteEscaper implements Escaper {
    public static final QuoteEscaper instance = new QuoteEscaper();

    public QuoteEscaper() {}

    public String applyString(String x) {
        return "(" + x + ")";
    }
}
