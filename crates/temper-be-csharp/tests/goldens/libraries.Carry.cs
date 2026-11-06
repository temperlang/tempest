namespace Libraries;

public class Carry
{
    private readonly bool carrying;
    private readonly int state;

    public Carry(int start)
    {
        this.carrying = false;
        this.state = 0;
        if (start != 0)
        {
            this.carrying = true;
            this.state = start;
        }
    }

    public bool Carrying
    {
        get
        {
            return this.carrying;
        }
    }

    public int State
    {
        get
        {
            return this.state;
        }
    }
}
