package angle_call;

import java.util.List;

final class Picker {
    public Picker() {}

    public <T> T pick(List<T> things) {
        return AngleCallGlobal.lastOf(things);
    }
}
