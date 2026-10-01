pub class A {
    x: usize = 0,
}

class A {
    fn bump() {
        a.x += 1;
    }
}

class A::Counter {
    fn bump2() {
        a.x += 1;
    }
}

fn mk_vec() -> Vector<usize> {
    let v: Vector<usize> = [];
    v.push_back(1);
    let w: Vector<usize> = Vector<usize>[3];
    return v;
}

fn main() -> int {
    return 0;
}
