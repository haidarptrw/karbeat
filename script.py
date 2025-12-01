import os
import shutil

'''
Handle the script for install cargo, so we don't have to navigate through rust folder to modify Cargo
'''
def main():
    rust_directory = os.path.join("rust")
    
    # execute the rest of args 