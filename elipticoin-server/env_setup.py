# python
import re
import os
import errno
from os import environ

class bcolors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'
    
# Go through each line of keys.txt and strip keys and turn into variables v_key(public) & p_key(private)
file1 = open('keys.txt', 'r')
Lines = file1.readlines() 
count = 0
for line in Lines: 
    if count == 0:
        v_key = line.strip().replace('Verification Key (Address): ', '')
    elif count == 1:
        p_key = line.strip().replace('Full Private Key: ', '')
    count += 1

# If PRIVATE_KEY exists and environement variable then set p_key to environment variable PRIVATE_KEY
if environ.get('PRIVATE_KEY') is not None and len(environ.get('PRIVATE_KEY')) > 0:
    p_key = environ.get('PRIVATE_KEY')
    print(bcolors.WARNING + 'Private Key found in ENV: ' + p_key  + bcolors.ENDC)
else:
    print(bcolors.WARNING + 'Generated Public Key: ' + v_key  + bcolors.ENDC)
    print(bcolors.WARNING + 'Generated Private Key: ' + p_key  + bcolors.ENDC)

# Save p_key to .env
sample_env = open('.env.sample', 'r')
replace_env_txt = sample_env.read().replace('PRIVATE_KEY=', 'PRIVATE_KEY=' + p_key)
f = open(".env", "a")
f.write(replace_env_txt)
f.close()



# filename = "/shared/private_key.txt"
# if not os.path.exists('/shared'):
#     os.makedirs('/shared')

# completeName = os.path.join("/shared", "priv_key.txt")         
# file1 = open(completeName, "w")
# file1.write(p_key)
# file1.close()