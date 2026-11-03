package property_members;

final class Low3 implements Mid {
    public Low3() {}

    public String toString() {
        return "Low3";
    }

    public boolean eq(High other) {
        return other instanceof Low3;
    }
}
