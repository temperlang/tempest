package private_method;

final class Label {
    private final int end;

    public Label(int end) {
        this.end = end;
    }

    public String getText() {
        return "label";
    }

    public int close(Sink sink) {
        return sink.end() + this.end;
    }

    public String noted(String more) {
        return new Note(this.getText()).text(more).body;
    }
}
