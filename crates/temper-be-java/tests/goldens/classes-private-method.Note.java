package private_method;

final class Note {
    public final String body;

    public Note(String body) {
        this.body = body;
    }

    public Note text(String more) {
        return new Note(this.body + more);
    }

    public String getBody() {
        return this.body;
    }
}
