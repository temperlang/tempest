package content_tags;

public final class AppendParseEffect implements Effect {
    public final String text;

    public AppendParseEffect(String text) {
        this.text = text;
    }

    public void enactOn(Buffer c) {
        c.append(this.text);
    }

    public String getText() {
        return this.text;
    }
}
