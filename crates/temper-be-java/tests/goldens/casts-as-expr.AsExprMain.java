package as_expr;

import temper.core.Core;

public final class AsExprMain {
    private AsExprMain() {}

    public static void main(String[] args) throws ClassNotFoundException {
        Core.initSimpleLogging();
        Class.forName("as_expr.AsExprGlobal");
        Core.waitUntilTasksComplete();
    }
}
