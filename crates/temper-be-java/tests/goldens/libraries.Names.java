package libraries;

public final class Names implements Sink<Tag> {
    public Names() {}

    public int take(Tag e) {
        if (e.name.equals("abc")) {
            return 3;
        } else {
            return 0;
        }
    }
}
