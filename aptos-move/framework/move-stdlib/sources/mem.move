/// Module with methods for safe memory manipulation.
/// I.e. swapping/replacing non-copyable/non-droppable types.
module std::mem {
    /// Swap contents of two passed mutable references.
    public native fun swap<T>(left: &mut T, right: &mut T);

    /// Replace value reference points to with the given new value,
    /// and return value it had before.
    public fun replace<T>(ref: &mut T, new: T): T {
        swap(ref, &mut new);
        new
    }

   spec swap<T>(left: &mut T, right: &mut T) {
        pragma opaque;
        aborts_if false;
        ensures right == old(left);
        ensures left == old(right);
    }

    spec replace<T>(ref: &mut T, new: T): T {
        pragma opaque;
        aborts_if false;
        ensures result == old(ref);
        ensures ref == new;
    }

    // tests

    #[test_only]
    use std::vector;

    #[test]
    fun test_swap_ints() {
        let a = 1;
        let b = 2;
        let v = vector[3, 4, 5, 6];

        swap(&mut a, &mut b);
        assert!(a == 2, 0);
        assert!(b == 1, 1);

        swap(&mut a, vector::borrow_mut(&mut v, 0));
        assert!(a == 3, 0);
        assert!(vector::borrow(&v, 0) == &2, 1);

        swap(vector::borrow_mut(&mut v, 2), &mut a);
        assert!(a == 5, 0);
        assert!(vector::borrow(&v, 2) == &3, 1);
    }

    #[test_only]
    struct SomeStruct has drop {
        f: u64,
        v: vector<u64>,
    }

    #[test]
    fun test_swap_struct() {
        let a = 1;
        let s1 = SomeStruct { f: 2, v: vector[3, 4]};
        let s2 = SomeStruct { f: 5, v: vector[6, 7]};
        let vs = vector[SomeStruct { f: 8, v: vector[9, 10]}, SomeStruct { f: 11, v: vector[12, 13]}];


        swap(&mut s1, &mut s2);
        assert!(&s1 == &SomeStruct { f: 5, v: vector[6, 7]}, 0);
        assert!(&s2 == &SomeStruct { f: 2, v: vector[3, 4]}, 1);

        swap(&mut s1.f, &mut a);
        assert!(s1.f == 1, 2);
        assert!(a == 5, 3);

        swap(&mut s1.f, vector::borrow_mut(&mut s1.v, 0));
        assert!(s1.f == 6, 4);
        assert!(vector::borrow(&s1.v, 0) == &1, 5);

        swap(&mut s2, vector::borrow_mut(&mut vs, 0));
        assert!(&s2 == &SomeStruct { f: 8, v: vector[9, 10]}, 6);
        assert!(vector::borrow(&vs, 0) == &SomeStruct { f: 2, v: vector[3, 4]}, 7);
    }
}
