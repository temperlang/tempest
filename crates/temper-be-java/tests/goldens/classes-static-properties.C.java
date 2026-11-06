package static_properties;

final class C {
    public C() {}

    public static void sayHi() {
        StaticPropertiesGlobal.console.log(compose("C", "'hi.'"));
    }

    static String compose(String subject, String message) {
        return subject + " says " + message;
    }
}
