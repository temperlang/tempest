namespace WriteBackThroughField;

internal class Pong
{
    public Pong()
    {
    }

    public void Pong_(Ping a)
    {
        {
            Ping o = a;
            o.N = o.N + 1;
        }
    }

    public int Bump(Ping a, int by)
    {
        {
            Ping o = a;
            o.N = o.N + by;
        }
        return a.N;
    }
}
