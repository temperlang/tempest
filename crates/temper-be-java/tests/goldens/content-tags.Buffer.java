package content_tags;

public final class Buffer {
    private final StringBuilder sb;

    public Buffer() {
        this.sb = new StringBuilder();
    }

    public void append(String x) {
        this.sb.append(x);
    }

    public void mark(Mark m) {
        this.sb.append("{" + m.label + "}");
    }

    public String toString() {
        return this.sb.toString();
    }
}
