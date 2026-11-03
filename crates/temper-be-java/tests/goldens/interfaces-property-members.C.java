package property_members;

final class C implements J {
    public String x;

    public C(String x) {
        this.x = x;
    }

    public String getX() {
        return this.x;
    }

    public void setX(String newX) {
        this.x = newX;
    }
}
