from random import randint


class LilypadDemo:
    def __init__(self, msg: str) -> None:
        self.msg = msg
        
    def print_message(self, extra_stuff: bool):
        if extra_stuff:
            print("Your number is " + str(randint(0, 5)))
        print(self.msg)

x = LilypadDemo("Go Gators!")
x.print_message(True)
