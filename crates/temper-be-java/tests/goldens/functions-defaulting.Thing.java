package defaulting;

import temper.core.Nullable;

final class Thing {
    public Thing() {}

    public void say(@Nullable String a, @Nullable String b) {
        String a2 = a != null ? a : "";
        String b2 = b != null ? b : "";
        DefaultingGlobal.console.log("saying " + a2 + b2);
    }

    public void say(@Nullable String a) {
        say(a, null);
    }

    public void say() {
        say(null, null);
    }

    public static void sayStatic(String a, @Nullable String b) {
        String b2 = b != null ? b : "";
        DefaultingGlobal.console.log("static " + a + b2);
    }

    public static void sayStatic(String a) {
        sayStatic(a, null);
    }
}
