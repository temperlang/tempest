namespace Nullables;

public class Node
{
    private readonly int value;
    private readonly Node? next;

    public Node(int value, Node? next)
    {
        this.value = value;
        this.next = next;
    }

    public int Value
    {
        get
        {
            return this.value;
        }
    }

    public Node? Next
    {
        get
        {
            return this.next;
        }
    }

    public int Last()
    {
        Node? n = this.next;
        if (n == null)
        {
            return this.value;
        }
        else
        {
            return n.Last();
        }
    }
}
