# Example Python Source File for Testing

#Testing multiple comments in a row
# Comment 1
# Comment 2
# Comment 3

#Testing imports
import random 
import math

#Testing multiple variable declarations
count1 = 0
count2 = 100
numbers = []
evens = []
odds = []

#Testing for loop code block
for count1 in range(count2):
    numbers += [count1]
    #Testing multiple if else statements
    if count1 % 2 == 0:
        evens += [count1]
    else:
        odds += [count1]
        
    if count1 == 99:
        print("Done.")
    
#Testing multiple print statements in a row
print("All numbers: ", numbers)
print("Even numbers: ", evens)
print("Odd numbers: ", odds)

i = 0
#Testing while loop
while i < 10:
    i += 1
    if i == 9:
        print("Done.")
    elif i == 8:
        print("Almost Done.")
    else:
        continue

#Testing def 
def chooseStarterType(starterType): 
    if starterType == "fire":
        print("You chose Chimchar!")
    elif starterType == "water":
        print ("You chose Piplup!")
    elif starterType == "grass":
        print("You chose Turtwig!")
    else:
        print("You chose Pikachu!")

chooseStarterType("fire")
chooseStarterType("any")

#Testing python dictionaries
dict1 = {
    "fire": "Torchic",
    "water": "Mudkip",
    "grass": "Treecko"
}
print(dict1)

#Testing try blocks
try:
  print(x)
except:
  print("Something went wrong")
else:
  print("Nothing went wrong")
finally:
  print("The 'try except' is finished")

x = "Works."
try:
  print(x)
except:
  print("Something went wrong")
else:
  print("Nothing went wrong")
finally:
  print("The 'try except' is finished")
