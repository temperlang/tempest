namespace WriteBackThroughField;

internal class Ping
{
    private readonly Pong peer;
    private int n;
    private int most;

    public Ping(Pong peer)
    {
        this.peer = peer;
        this.n = 0;
        this.most = 0;
    }

    public Pong Peer
    {
        get
        {
            return this.peer;
        }
    }

    public int N
    {
        get
        {
            return this.n;
        }
        set
        {
            this.n = value;
        }
    }

    public int Most
    {
        get
        {
            return this.most;
        }
        set
        {
            this.most = value;
        }
    }

    public int Ping_()
    {
        this.peer.Pong_(this);
        return this.n;
    }

    public int Pass()
    {
        return this.peer.Bump(this, this.n);
    }

    public bool Record()
    {
        this.n = this.n + 1;
        if (this.n > this.most)
        {
            this.most = this.n;
        }
        return this.n > this.most - 1;
    }

    public int Around()
    {
        return this.n + this.peer.Bump(this, 10) + this.n;
    }
}
