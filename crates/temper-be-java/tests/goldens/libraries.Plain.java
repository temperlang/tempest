package libraries;

public final class Plain<T> implements Effect<T> {
    public final String text;

    public Plain(String text) {
        this.text = text;
    }

    public String getText() {
        return this.text;
    }
}
