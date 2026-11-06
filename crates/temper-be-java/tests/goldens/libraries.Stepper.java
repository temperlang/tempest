package libraries;

import java.util.List;

public interface Stepper<T> {
    List<T> step();
}
