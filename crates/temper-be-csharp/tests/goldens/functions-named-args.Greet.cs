namespace NamedArgs;

public class Greet
{
    public Greet(string? message = null, bool? repeat = null)
    {
        string message2 = message ?? "Hi!";
        bool repeat2 = repeat ?? false;
        NamedArgsGlobal.console.Log(message2);
        if (repeat2)
        {
            NamedArgsGlobal.console.Log(message2);
        }
    }
}
