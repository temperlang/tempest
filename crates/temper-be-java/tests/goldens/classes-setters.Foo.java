package setters;

final class Foo {
    private String backing;

    public Foo(String backing) {
        this.backing = backing;
    }

    public String getBar() {
        return this.backing;
    }

    public void setBar(String value) {
        this.backing = value;
    }
}
