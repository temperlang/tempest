package setters;

final class Bar {
    private String backing;

    public Bar(String backing) {
        this.backing = backing;
    }

    public String getBar() {
        return this.backing;
    }

    private void setBar(String value) {
        this.backing = value;
    }

    public void usingSet(String value) {
        this.setBar(value);
    }
}
